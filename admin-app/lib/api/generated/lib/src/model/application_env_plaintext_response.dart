//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_env_plaintext_response.g.dart';

/// ApplicationEnvPlaintextResponse
///
/// Properties:
/// * [applicationId]
/// * [content]
/// * [digest]
/// * [envVersion]
/// * [fileName]
/// * [format]
/// * [id]
/// * [module]
/// * [updatedAt]
/// * [version]
@BuiltValue()
abstract class ApplicationEnvPlaintextResponse implements Built<ApplicationEnvPlaintextResponse, ApplicationEnvPlaintextResponseBuilder> {
  @BuiltValueField(wireName: r'application_id')
  String get applicationId;

  @BuiltValueField(wireName: r'content')
  String get content;

  @BuiltValueField(wireName: r'digest')
  String get digest;

  @BuiltValueField(wireName: r'env_version')
  int get envVersion;

  @BuiltValueField(wireName: r'file_name')
  String get fileName;

  @BuiltValueField(wireName: r'format')
  String get format;

  @BuiltValueField(wireName: r'id')
  String get id;

  @BuiltValueField(wireName: r'module')
  String get module;

  @BuiltValueField(wireName: r'updated_at')
  String get updatedAt;

  @BuiltValueField(wireName: r'version')
  int get version;

  ApplicationEnvPlaintextResponse._();

  factory ApplicationEnvPlaintextResponse([void updates(ApplicationEnvPlaintextResponseBuilder b)]) = _$ApplicationEnvPlaintextResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationEnvPlaintextResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationEnvPlaintextResponse> get serializer => _$ApplicationEnvPlaintextResponseSerializer();
}

class _$ApplicationEnvPlaintextResponseSerializer implements PrimitiveSerializer<ApplicationEnvPlaintextResponse> {
  @override
  final Iterable<Type> types = const [ApplicationEnvPlaintextResponse, _$ApplicationEnvPlaintextResponse];

  @override
  final String wireName = r'ApplicationEnvPlaintextResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationEnvPlaintextResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'application_id';
    yield serializers.serialize(
      object.applicationId,
      specifiedType: const FullType(String),
    );
    yield r'content';
    yield serializers.serialize(
      object.content,
      specifiedType: const FullType(String),
    );
    yield r'digest';
    yield serializers.serialize(
      object.digest,
      specifiedType: const FullType(String),
    );
    yield r'env_version';
    yield serializers.serialize(
      object.envVersion,
      specifiedType: const FullType(int),
    );
    yield r'file_name';
    yield serializers.serialize(
      object.fileName,
      specifiedType: const FullType(String),
    );
    yield r'format';
    yield serializers.serialize(
      object.format,
      specifiedType: const FullType(String),
    );
    yield r'id';
    yield serializers.serialize(
      object.id,
      specifiedType: const FullType(String),
    );
    yield r'module';
    yield serializers.serialize(
      object.module,
      specifiedType: const FullType(String),
    );
    yield r'updated_at';
    yield serializers.serialize(
      object.updatedAt,
      specifiedType: const FullType(String),
    );
    yield r'version';
    yield serializers.serialize(
      object.version,
      specifiedType: const FullType(int),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationEnvPlaintextResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationEnvPlaintextResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'application_id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.applicationId = valueDes;
          break;
        case r'content':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.content = valueDes;
          break;
        case r'digest':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.digest = valueDes;
          break;
        case r'env_version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.envVersion = valueDes;
          break;
        case r'file_name':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.fileName = valueDes;
          break;
        case r'format':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.format = valueDes;
          break;
        case r'id':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.id = valueDes;
          break;
        case r'module':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.module = valueDes;
          break;
        case r'updated_at':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(String),
          ) as String;
          result.updatedAt = valueDes;
          break;
        case r'version':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(int),
          ) as int;
          result.version = valueDes;
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationEnvPlaintextResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationEnvPlaintextResponseBuilder();
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
