//
// AUTO-GENERATED FILE, DO NOT MODIFY!
//

// ignore_for_file: unused_element
import 'package:built_collection/built_collection.dart';
import 'package:deploy_go_api_client/src/model/application_config_file_response.dart';
import 'package:built_value/built_value.dart';
import 'package:built_value/serializer.dart';

part 'application_config_file_list_response.g.dart';

/// ApplicationConfigFileListResponse
///
/// Properties:
/// * [items]
@BuiltValue()
abstract class ApplicationConfigFileListResponse implements Built<ApplicationConfigFileListResponse, ApplicationConfigFileListResponseBuilder> {
  @BuiltValueField(wireName: r'items')
  BuiltList<ApplicationConfigFileResponse> get items;

  ApplicationConfigFileListResponse._();

  factory ApplicationConfigFileListResponse([void updates(ApplicationConfigFileListResponseBuilder b)]) = _$ApplicationConfigFileListResponse;

  @BuiltValueHook(initializeBuilder: true)
  static void _defaults(ApplicationConfigFileListResponseBuilder b) => b;

  @BuiltValueSerializer(custom: true)
  static Serializer<ApplicationConfigFileListResponse> get serializer => _$ApplicationConfigFileListResponseSerializer();
}

class _$ApplicationConfigFileListResponseSerializer implements PrimitiveSerializer<ApplicationConfigFileListResponse> {
  @override
  final Iterable<Type> types = const [ApplicationConfigFileListResponse, _$ApplicationConfigFileListResponse];

  @override
  final String wireName = r'ApplicationConfigFileListResponse';

  Iterable<Object?> _serializeProperties(
    Serializers serializers,
    ApplicationConfigFileListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) sync* {
    yield r'items';
    yield serializers.serialize(
      object.items,
      specifiedType: const FullType(BuiltList, [FullType(ApplicationConfigFileResponse)]),
    );
  }

  @override
  Object serialize(
    Serializers serializers,
    ApplicationConfigFileListResponse object, {
    FullType specifiedType = FullType.unspecified,
  }) {
    return _serializeProperties(serializers, object, specifiedType: specifiedType).toList();
  }

  void _deserializeProperties(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
    required List<Object?> serializedList,
    required ApplicationConfigFileListResponseBuilder result,
    required List<Object?> unhandled,
  }) {
    for (var i = 0; i < serializedList.length; i += 2) {
      final key = serializedList[i] as String;
      final value = serializedList[i + 1];
      switch (key) {
        case r'items':
          final valueDes = serializers.deserialize(
            value,
            specifiedType: const FullType(BuiltList, [FullType(ApplicationConfigFileResponse)]),
          ) as BuiltList<ApplicationConfigFileResponse>;
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
  ApplicationConfigFileListResponse deserialize(
    Serializers serializers,
    Object serialized, {
    FullType specifiedType = FullType.unspecified,
  }) {
    final result = ApplicationConfigFileListResponseBuilder();
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
