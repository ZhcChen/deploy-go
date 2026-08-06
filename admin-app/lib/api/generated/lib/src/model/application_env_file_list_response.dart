//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/application_env_file_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_env_file_list_response.g.dart';

/// ApplicationEnvFileListResponse
///
/// Properties:
/// * [items]
@BuiltValue()
abstract class ApplicationEnvFileListResponse implements Built<ApplicationEnvFileListResponse, ApplicationEnvFileListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<ApplicationEnvFileResponse> get items;

  ApplicationEnvFileListResponse._();

  factory ApplicationEnvFileListResponse([void updates(ApplicationEnvFileListResponseBuilder b)]) = _$ApplicationEnvFileListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationEnvFileListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationEnvFileListResponse> get serializer => _$ApplicationEnvFileListResponseSerializer();
}

class _$ApplicationEnvFileListResponseSerializer implements PrimitiveSerializer<ApplicationEnvFileListResponse> {
  @override
  final Iterable<Type> types = const [ApplicationEnvFileListResponse, _$ApplicationEnvFileListResponse];

  @override
  final String wireName = r'ApplicationEnvFileListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationEnvFileListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(ApplicationEnvFileResponse)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationEnvFileListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationEnvFileListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ApplicationEnvFileResponse)]),
          ) as BuiltList<ApplicationEnvFileResponse>;
          result.items.replace(valueDes);
          break;
        default:
          unhandled.add(key);
          unhandled.add(value);
          break;
      }
    }
  }

  @override
  ApplicationEnvFileListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationEnvFileListResponseBuilder();
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
