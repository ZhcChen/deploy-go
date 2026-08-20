//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:deploy_go_api_client/src/model/image_template.dart';
import 'package:built_collection/built_collection.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'image_deploy_spec.g.dart';

/// ImageDeploySpec
///
/// Properties:
/// * [envFiles]
/// * [hostPort]
/// * [image]
/// * [template]
@BuiltValue()
abstract class ImageDeploySpec implements Built<ImageDeploySpec, ImageDeploySpecBuilder> {
  @BuiltValueField(wireName: r'env_files')
  BuiltList<String> get envFiles;

  @BuiltValueField(wireName: r'host_port')
  int get hostPort;

  @BuiltValueField(wireName: r'image')
  String get image;

  @BuiltValueField(wireName: r'template')
  ImageTemplate get template;
  // enum templateEnum {  redis,  valkey,  postgres,  etcd,  };

  ImageDeploySpec._();

  factory ImageDeploySpec([void updates(ImageDeploySpecBuilder b)]) = _$ImageDeploySpec;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ImageDeploySpecBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ImageDeploySpec> get serializer => _$ImageDeploySpecSerializer();
}

class _$ImageDeploySpecSerializer implements PrimitiveSerializer<ImageDeploySpec> {
  @override
  final Iterable<Type> types = const [ImageDeploySpec, _$ImageDeploySpec];

  @override
  final String wireName = r'ImageDeploySpec';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ImageDeploySpec object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'env_files';
    yield serializers.serialize(
      object.envFiles,
      specifiedType: const FullType(BuiltList, [FullType(String)]),
    );
    yield r'host_port';
    yield serializers.serialize(
      object.hostPort,
      specifiedType: const FullType(int),
    );
    yield r'image';
    yield serializers.serialize(
      object.image,
      specifiedType: const FullType(String),
    );
    yield r'template';
    yield serializers.serialize(
      object.template,
      specifiedType: const FullType(ImageTemplate),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ImageDeploySpec object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ImageDeploySpecBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'env_files':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(String)]),
          ) as BuiltList<String>;
          result.envFiles.replace(valueDes);
          break;
        case r'host_port':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.hostPort = valueDes;
          break;
        case r'image':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.image = valueDes;
          break;
        case r'template':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(ImageTemplate),
          ) as ImageTemplate;
          result.template = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ImageDeploySpec deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ImageDeploySpecBuilder();
    final serializedList = (serialized as Iterable<Object?>).toList();
    final unhandled = <Object?>[];
    _deserializeProperties(
      serializers,
      serialized,
      specifiedType: specifiedType,
      serializedList: serializedList,
      unhandled: unhandled,
      result: result,
    );
    return result.build();
  }
}
